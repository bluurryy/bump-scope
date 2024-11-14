inspect_asm::vec_map::grow:
	push rbp
	push r15
	push r14
	push r13
	push r12
	push rbx
	push rax
	mov r14, rdi
	mov rbx, qword ptr [rsi + 24]
	mov r12, qword ptr [rsi]
	mov rbp, qword ptr [rsi + 8]
	mov r13, qword ptr [rsi + 16]
	test rbp, rbp
	je .LBB0_5
	mov rax, rbp
	shr rax, 60
	jne .LBB0_10
	lea r15, [8*rbp]
	mov rcx, qword ptr [rbx]
	mov rax, qword ptr [rcx]
	dec rax
	and rax, -8
	lea rsi, [r15 + 8]
	add rsi, rax
	mov rdx, -1
	cmovae rdx, rsi
	cmp rdx, qword ptr [rcx + 8]
	ja .LBB0_0
	mov qword ptr [rcx], rdx
	add rax, 8
	jne .LBB0_1
.LBB0_0:
	mov esi, 8
	mov rdi, rbx
	mov rdx, r15
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::alloc_in_another_chunk
	test rax, rax
	je .LBB0_11
.LBB0_1:
	movabs rdx, 4611686018427387903
	lea rcx, [rbp - 1]
	mov rdi, rcx
	and rdi, rdx
	cmp rdi, rcx
	cmovae rdi, rcx
	cmp rdi, 18
	jbe .LBB0_2
	lea rsi, [rdx + rbp]
	and rsi, rdx
	cmp rsi, rcx
	cmovae rsi, rcx
	lea rdx, [r12 + 4*rsi]
	add rdx, 4
	cmp rax, rdx
	jae .LBB0_8
	lea rdx, [rax + 8*rsi]
	add rdx, 8
	cmp r12, rdx
	jae .LBB0_8
.LBB0_2:
	xor edx, edx
	mov rsi, r12
.LBB0_3:
	lea rdi, [r12 + 4*rbp]
.LBB0_4:
	mov r8, rdx
	mov edx, dword ptr [rsi]
	mov qword ptr [rax + 8*r8], rdx
	lea rdx, [r8 + 1]
	cmp rcx, r8
	je .LBB0_6
	add rsi, 4
	cmp rsi, rdi
	jne .LBB0_4
	jmp .LBB0_6
.LBB0_5:
	mov eax, 8
	xor edx, edx
.LBB0_6:
	lea rsi, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rsi, qword ptr [rcx]
	jne .LBB0_7
	mov qword ptr [rcx], r12
.LBB0_7:
	mov qword ptr [r14], rax
	mov qword ptr [r14 + 8], rdx
	mov qword ptr [r14 + 16], rbp
	mov qword ptr [r14 + 24], rbx
	mov rax, r14
	add rsp, 8
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_8:
	inc rdi
	movabs rdx, 9223372036854775804
	and rdx, rdi
	lea rsi, [r12 + 4*rdx]
	xor r8d, r8d
	xorps xmm0, xmm0
.LBB0_9:
	movsd xmm1, qword ptr [r12 + 4*r8]
	movsd xmm2, qword ptr [r12 + 4*r8 + 8]
	unpcklps xmm1, xmm0
	unpcklps xmm2, xmm0
	movups xmmword ptr [rax + 8*r8], xmm1
	movups xmmword ptr [rax + 8*r8 + 16], xmm2
	add r8, 4
	cmp rdx, r8
	jne .LBB0_9
	cmp rdi, rdx
	jne .LBB0_3
	jmp .LBB0_6
.LBB0_10:
	call qword ptr [rip + bump_scope::bump_allocator::invalid_slice_layout@GOTPCREL]
	jmp .LBB0_12
.LBB0_11:
	mov edi, 8
	mov rsi, r15
	call qword ptr [rip + alloc::alloc::handle_alloc_error@GOTPCREL]
.LBB0_12:
	ud2
	lea rdx, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_13
	mov qword ptr [rcx], r12
.LBB0_13:
	mov rdi, rax
	call _Unwind_Resume@PLT
