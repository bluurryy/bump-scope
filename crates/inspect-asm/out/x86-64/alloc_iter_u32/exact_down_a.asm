inspect_asm::alloc_iter_u32::exact_down_a:
	push r15
	push r14
	push rbx
	mov rax, rdx
	shr rax, 61
	jne .LBB0_7
	lea rbx, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov r8, rax
	sub r8, qword ptr [rcx + 8]
	cmp rbx, r8
	ja .LBB0_5
	sub rax, rbx
	mov qword ptr [rcx], rax
	test rdx, rdx
	je .LBB0_4
.LBB0_0:
	add rbx, -4
	shr rbx, 2
	cmp rdx, rbx
	cmovb rbx, rdx
	mov rcx, rsi
	mov rdi, rax
	cmp rbx, 8
	jb .LBB0_2
	mov r8, rax
	sub r8, rsi
	mov rcx, rsi
	mov rdi, rax
	cmp r8, 31
	jbe .LBB0_2
	inc rbx
	mov ecx, ebx
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	sub rbx, rdi
	lea rcx, [rsi + 4*rbx]
	lea rdi, [rax + 4*rbx]
	xor r8d, r8d
.LBB0_1:
	movups xmm0, xmmword ptr [rsi + 4*r8]
	movups xmm1, xmmword ptr [rsi + 4*r8 + 16]
	movups xmmword ptr [rax + 4*r8], xmm0
	movups xmmword ptr [rax + 4*r8 + 16], xmm1
	add r8, 8
	cmp rbx, r8
	jne .LBB0_1
.LBB0_2:
	lea rsi, [rsi + 4*rdx]
	lea r8, [rax + 4*rdx]
.LBB0_3:
	cmp rcx, rsi
	je .LBB0_6
	mov r9d, dword ptr [rcx]
	add rcx, 4
	mov dword ptr [rdi], r9d
	add rdi, 4
	cmp rdi, r8
	jne .LBB0_3
.LBB0_4:
	pop rbx
	pop r14
	pop r15
	ret
.LBB0_5:
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, r14
	mov rdx, r15
	test rdx, rdx
	jne .LBB0_0
	jmp .LBB0_4
.LBB0_6:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
.LBB0_7:
	call qword ptr [rip + bump_scope::private::capacity_overflow@GOTPCREL]
	mov rdi, rax
	call _Unwind_Resume@PLT
