inspect_asm::alloc_iter_u32::try_up_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	test rdx, rdx
	je .LBB_1
	mov rax, rdx
	shr rax, 61
	je .LBB_3
.LBB_5:
	xor eax, eax
	jmp .LBB_13
.LBB_1:
	mov eax, 4
	xor edx, edx
	jmp .LBB_13
.LBB_3:
	mov rbx, rsi
	lea r15, [4*rdx]
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rsi, qword ptr [rcx + 8]
	sub rsi, rax
	cmp r15, rsi
	ja .LBB_4
	lea rsi, [r15 + rax]
	mov qword ptr [rcx], rsi
.LBB_7:
	mov qword ptr [rsp], rax
	mov qword ptr [rsp + 8], 0
	mov qword ptr [rsp + 16], rdx
	mov qword ptr [rsp + 24], rdi
	xor r12d, r12d
	mov r14, rsp
	xor edx, edx
	jmp .LBB_8
.LBB_11:
	mov dword ptr [rax + 4*rdx], ebp
	inc rdx
	mov qword ptr [rsp + 8], rdx
	add r12, 4
	cmp r15, r12
	je .LBB_12
.LBB_8:
	mov ebp, dword ptr [rbx + r12]
	cmp qword ptr [rsp + 16], rdx
	jne .LBB_11
	mov rdi, r14
	call bump_scope::bump_vec::BumpVec<T,A,_,_>::generic_grow_cold
	test al, al
	jne .LBB_5
	mov rax, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 8]
	jmp .LBB_11
.LBB_12:
	mov rax, qword ptr [rsp]
.LBB_13:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB_4:
	mov r14, rdi
	mov rsi, rdx
	mov r12, rdx
	call bump_scope::bump_scope::BumpScope<A,_,_>::do_alloc_slice_in_another_chunk
	mov rdi, r14
	mov rdx, r12
	test rax, rax
	jne .LBB_7
	jmp .LBB_5