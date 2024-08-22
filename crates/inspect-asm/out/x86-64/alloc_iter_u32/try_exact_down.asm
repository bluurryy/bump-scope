inspect_asm::alloc_iter_u32::try_exact_down:
	push r15
	push r14
	push rbx
	mov rax, rdx
	shr rax, 61
	je .LBB0_1
	xor eax, eax
.LBB0_0:
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_1:
	lea rbx, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r8, rax
	sub r8, qword ptr [rcx + 8]
	cmp rbx, r8
	ja .LBB0_6
	sub rax, rbx
	and rax, -4
	mov qword ptr [rcx], rax
	je .LBB0_6
.LBB0_2:
	test rdx, rdx
	je .LBB0_0
	add rbx, -4
	shr rbx, 2
	cmp rdx, rbx
	cmovb rbx, rdx
	mov rcx, rax
	mov rdi, rsi
	cmp rbx, 8
	jb .LBB0_4
	mov r8, rax
	sub r8, rsi
	mov rcx, rax
	mov rdi, rsi
	cmp r8, 32
	jb .LBB0_4
	inc rbx
	mov ecx, ebx
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	sub rbx, rdi
	lea rcx, [rax + 4*rbx]
	lea rdi, [rsi + 4*rbx]
	xor r8d, r8d
.LBB0_3:
	movups xmm0, xmmword ptr [rsi + 4*r8]
	movups xmm1, xmmword ptr [rsi + 4*r8 + 16]
	movups xmmword ptr [rax + 4*r8], xmm0
	movups xmmword ptr [rax + 4*r8 + 16], xmm1
	add r8, 8
	cmp rbx, r8
	jne .LBB0_3
.LBB0_4:
	lea rsi, [rsi + 4*rdx]
	lea r8, [rax + 4*rdx]
.LBB0_5:
	cmp rdi, rsi
	je .LBB0_7
	mov r9d, dword ptr [rdi]
	add rdi, 4
	mov dword ptr [rcx], r9d
	add rcx, 4
	cmp rcx, r8
	jne .LBB0_5
	jmp .LBB0_0
.LBB0_6:
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov rdx, r15
	test rax, rax
	jne .LBB0_2
	xor eax, eax
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_7:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
	mov rdi, rax
	call _Unwind_Resume@PLT
