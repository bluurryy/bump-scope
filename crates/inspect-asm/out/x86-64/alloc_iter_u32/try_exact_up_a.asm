inspect_asm::alloc_iter_u32::try_exact_up_a:
	push r14
	push rbx
	push rax
	mov rax, rdx
	shr rax, 61
	jne .LBB0_6
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi]
	mov rax, qword ptr [r8]
	mov r9, qword ptr [r8 + 8]
	sub r9, rax
	cmp rcx, r9
	ja .LBB0_5
	add rcx, rax
	mov qword ptr [r8], rcx
.LBB0_0:
	test rdx, rdx
	je .LBB0_4
	lea rcx, [rdx - 1]
	movabs r8, 4611686018427387903
	and r8, rcx
	cmp rdx, r8
	cmovb r8, rdx
	mov rcx, rax
	mov rdi, rsi
	cmp r8, 8
	jb .LBB0_2
	mov r9, rax
	sub r9, rsi
	mov rcx, rax
	mov rdi, rsi
	cmp r9, 32
	jb .LBB0_2
	inc r8
	mov ecx, r8d
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	sub r8, rdi
	lea rcx, [rax + 4*r8]
	lea rdi, [rsi + 4*r8]
	xor r9d, r9d
.LBB0_1:
	movups xmm0, xmmword ptr [rsi + 4*r9]
	movups xmm1, xmmword ptr [rsi + 4*r9 + 16]
	movups xmmword ptr [rax + 4*r9], xmm0
	movups xmmword ptr [rax + 4*r9 + 16], xmm1
	add r9, 8
	cmp r8, r9
	jne .LBB0_1
.LBB0_2:
	lea rsi, [rsi + 4*rdx]
	lea r8, [rax + 4*rdx]
.LBB0_3:
	cmp rdi, rsi
	je .LBB0_7
	mov r9d, dword ptr [rdi]
	add rdi, 4
	mov dword ptr [rcx], r9d
	add rcx, 4
	cmp rcx, r8
	jne .LBB0_3
.LBB0_4:
	test rax, rax
	je .LBB0_6
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB0_5:
	mov rbx, rsi
	mov rsi, rdx
	mov r14, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	mov rsi, rbx
	mov rdx, r14
	test rax, rax
	jne .LBB0_0
.LBB0_6:
	xor eax, eax
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB0_7:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
	mov rdi, rax
	call _Unwind_Resume@PLT
